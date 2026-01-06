
set @companieId='54609e06-cf0b-4501-9597-1a01b4bbf62a';
set @clinicId='';
set @startDay='2010-01-01';
set @endDay=CURDATE();

-- Dropping existing table if it exists
DROP TABLE IF EXISTS bi.report_bill_detail_temp;
CREATE TABLE bi.report_bill_detail_temp (
    `开单门店` VARCHAR(255),
    `病历编号` VARCHAR(255),
    `顾客姓名` VARCHAR(255),
    `性别` VARCHAR(10),
    `客户状态` VARCHAR(50),
    `咨询师` VARCHAR(255),
    `项目类型` VARCHAR(255),
    `项目分类` VARCHAR(255),
		`项目ID` VARCHAR(255),
    `开单项目` VARCHAR(255),
    `客户来源` VARCHAR(255),
    `规格` VARCHAR(255),
    `数量` INT,
    `单位` VARCHAR(50),
    `支付方式` VARCHAR(255),
    `实收金额` DECIMAL(15, 2),
    `开单人` VARCHAR(255),
    `业绩人` VARCHAR(255),
    `权益人` VARCHAR(255),
    `付款时间` DATETIME,
    `最早开单时间` DATETIME,
    `最后消耗时间` DATETIME,
    `开单时间` DATETIME,
    `备注` TEXT,
    `实质消费` DECIMAL(15, 2),
    `促销活动` VARCHAR(255),
    `组合项目` VARCHAR(255),
    `是否是网络订单` BOOLEAN,
    `是否是促销项目` BOOLEAN,
    `客户类型` INT,
    `开单提点类型` VARCHAR(255),
    `订单来源` INT,
    `是否是内购订单` INT
);

-- Step 3: Insert data into the table
INSERT INTO bi.report_bill_detail_temp (
    `开单门店`, `病历编号`, `顾客姓名`, `性别`, `客户状态`, `咨询师`,
    `项目类型`, `项目分类`, `项目ID`,`开单项目`, `客户来源`, `规格`, `数量`, `单位`,
    `支付方式`, `实收金额`, `开单人`, `业绩人`, `权益人`, `付款时间`,
    `最早开单时间`, `最后消耗时间`, `开单时间`, `备注`, `实质消费`,
    `促销活动`, `组合项目`, `是否是网络订单`, `是否是促销项目`, 
    `客户类型`, `开单提点类型`, `订单来源`, `是否是内购订单`
		)
select
/*
{"序号", "开单门店", "病历编号", "顾客姓名", "性别", "客户状态", "咨询师", "项目类型", "项目分类", "开单项目", "客户来源", "规格", "数量", "单位", "支付方式", "实收金额", "开单人", "业绩人", "权益人", "付款时间", "最早开单时间", "最后消耗时间", "开单时间", "备注"}

            headerTemp.add("实质消费");
            headerTemp.add("促销活动");
            headerTemp.add("组合项目");
            headerTemp.add("是否是网络订单");
            headerTemp.add("是否是促销项目");
            headerTemp.add("客户类型");
            headerTemp.add("开单提点类型");

*/
        /*billsdetail.**/
				
billsdetail.`bill_clinic_name` as `开单门店`,
billsdetail.`customer_cid` as `病历编号`,
billsdetail.`customer_name` as `顾客姓名`,
billsdetail.`customer_sex` as `性别`,
billsdetail.`visit_consumption_type` as `客户状态`,
billsdetail.`visit_consultant_name` as `咨询师`,
billsdetail.`item_type_name` as `项目类型`,
billsdetail.`item_category_name` as `项目分类`,
billsdetail.`item_id` as `项目ID`,
billsdetail.`item_name` as `开单项目`,
billsdetail.`customer_source_name` as `客户来源`,
billsdetail.`item_specification` as `规格`,
billsdetail.`item_count` as `数量`,
billsdetail.`item_unit` as `单位`,
billsdetail.`payment_methods` as `支付方式`,
billsdetail.`item_payment_amount`/100 as `实收金额`,
billsdetail.`creator_name` as `开单人`,
billsdetail.`performers` as `业绩人`,
billsdetail.`customer_stakeholder_name` as `权益人`,
billsdetail.`bill_pay_time` as `付款时间`,
-- billsdetail.`first_bill_time` as `最早开单时间`,
customer_exitr.`first_real_pay_time` as `最早开单时间`,
billsdetail.`last_course_detail_time` as `最后消耗时间`,
billsdetail.`bill_create_time` as `开单时间`,
billsdetail.`bill_memo` as `备注`,
IFNULL(billsdetail.`item_real_payment_amount`, 0)/100  - IFNULL(refunddetail.`refund_real_amount`, 0)/100 as `实质消费`,
-- refunddetail.`refund_real_amount`/100 as `退款实质消费`,
case billsdetail.`item_source_type` when 2 then billsdetail.`item_source_name` else '' end as `促销活动`,
case when billsdetail.`item_source_type`=1 and billsdetail.`item_source_service_is_group`=1 then billsdetail.`item_source_name` else '' end as `组合项目`,
billsdetail.`is_network_order` as `是否是网络订单`,
billsdetail.`Is_promotion` as `是否是促销项目`,
case billsdetail.`is_new_customer_bill` when 1 then 
	case when (
						billsdetail.`customer_source_id`='51e3824f-2d1c-c108-39b8-08d983bd5e69'
						or billsdetail.`customer_source_parent_id`='51e3824f-2d1c-c108-39b8-08d983bd5e69'
						or billsdetail.`customer_source_id`='574b3234-1dcf-cc13-966f-08d8c75173ec'
						or billsdetail.`customer_source_parent_id`='574b3234-1dcf-cc13-966f-08d8c75173ec'
						
						or billsdetail.`customer_source_parent_id`='8a9157ca-8c0c-c4b1-450d-08d6d364ea93'
						or billsdetail.`customer_source_parent_id`='fc84ad50-93a7-c51e-4449-08d5bbe55840'
						or billsdetail.`customer_source_parent_id`='6c774f32-57e2-ce8e-e3a6-08d666ee930e'
						or billsdetail.`customer_source_parent_id`='f02bdfc1-df5b-c7ec-ebf1-08da3c81c302'
						or billsdetail.`customer_source_parent_id`='e4ccdecb-17d4-c8f7-20a4-08da3c81b466'
						or billsdetail.`customer_source_parent_id`='2f185bd0-54ad-c8b1-a818-08da3c81e351'
						or billsdetail.`customer_source_parent_id`='aaabd8de-acaa-c250-72e1-08d8a70d7a2a'
						or billsdetail.`customer_source_parent_id`='16b733d2-2d84-c299-1db8-08d6da833452'
						)
						and billsdetail.`is_first_bill`=1
						and billsdetail.`first_day_total_real_payment_amount`>=200000
			then 2
			when (
						billsdetail.`customer_source_id`='ecd5d00a-080f-c705-3b6f-08d5bbe51a24'
						or billsdetail.`customer_source_parent_id`='ecd5d00a-080f-c705-3b6f-08d5bbe51a24'
						)
						and billsdetail.`is_new_customer_bill`=1
						and billsdetail.`is_network_order`=1
			then 0 end
	else 3 end as `客户类型`,
case billsdetail.`item_type` when 1 then billsdetail.`item_id` else billsdetail.`bill_classify_name` end as `开单提点类型`,
billsdetail.`bill_source` as `订单来源`,
billsdetail.`in_app_purchase` as `是否是内购订单`


        
        from
        ( 
         
         
         
             
        select
             
        paybook.`pay_book_id`,
        paybook.`bill_id`,
        paybook.`bill_clinic_id`,
        paybook.`bill_clinic_name`,
        paybook.`pay_clinic_id`,
        paybook.`pay_clinic_name`,
        paybook.`visit_consumption_type`,
        paybook.`visit_consultant_id`,
        paybook.`visit_consultant_name`,
        paybook.`payment_methods`,
        ifnull(paybook.`payment_check_off_code`,'') as `payment_check_off_code`,
        paybook.`bill_discounted_amount` as `bill_discounted_amount`,
        paybook.`bill_pay_amount` as `bill_pay_amount`,
        paybook.`total_payment_amount` as `bill_payment_amount`,
        paybook.`total_real_payment_amount` as `bill_real_payment_amount`,
        paybook.`total_balance_payment_amount` as `bill_balance_payment_amount`,
        paybook.`creator_id`,
        paybook.`creator_name`,
        paybook.`performers`,
        paybook.`performers_details`,
        paybook.`bill_create_time`,
        paybook.`bill_pay_time`,
     
             
        customer.`customer_id`,
        customer.`customer_name`,
        customer.`customer_cid`,
        customer.`customer_sex`,
        customer.`customer_card_id`,
        customer.`customer_card_name`,
        customer.`customer_source_id`,
        customer.`customer_source_name`,
        customer.`customer_source_parent_id`,
        customer.`first_bill_time`,
        customer.`last_course_detail_time`,
        customer.`customer_stakeholder_id`,
        customer.`customer_stakeholder_name`,
        customer.`nurse_id`,
        customer.`nurse_name`,
        customer.`max_new_customer_bill_time`,
        customer.`max_new_customer_time`,
     
             
        billdetail.`bill_detail_id`,
        billdetail.`item_id` as `item_source_id`,
        billdetail.`item_name` as `item_source_name`,
        billdetail.`care_type` as `item_source_type`,
        case billdetail.`care_type` when 0 then '服务项目' when 1 then '商品' else '促销活动' end as `item_source_type_name`,
        case billdetail.`care_type` when 0 then (select ifnull(cs.`IsGroupItem`, 0) as `item_source_service_is_group` from careservices cs where cs.`ID`=billdetail.`item_id`) else 0 end as `item_source_service_is_group`,
        billdetail.`is_network_order`,
        billdetail.`is_promotion`,
        ifnull(bill.`Memo`,'') as `bill_memo`,
     
             
        case when paybook.`bill_id`=customer.`first_bill_id` then ifnull((
            select
                sum(pbc.`PayAmount`) as `first_day_total_real_payment_amount`
            from
                paybooks pb
                left join bills b on b.`ID`=pb.`BillId`
                left join paybookchannels pbc on pbc.`PayBookId`=pb.`ID`
                left join paymentmethods pt on pt.`ID`=pbc.`ChannelId`
            where
                b.`Customer_ID`=customer.`customer_id`
                and DATE_FORMAT(pb.`PayTime`, '%Y-%m-%d')=DATE_FORMAT(customer.`first_bill_time`, '%Y-%m-%d')
                and (pt.`IsRealConsumption`=1 or pt.`ID`='6756de43-f712-c80b-9687-08d59e04cec0')
            group by
                b.`Customer_ID`
        ), 0) else 0 end as `first_day_total_real_payment_amount`,
     
             
        1 as `item_type`,
        '商品' as `item_type_name`,

        billproduct.`ID` as `bill_item_id`,
        good.`ID` as `item_id`,
        good.`Name` as `item_name`,
        goodscategory.`ID` as `item_category_id`,
        goodscategory.`Name` as `item_category_name`,
        goodscategoryparent.`ID` as `item_category_parent_id`,
        goodscategoryparent.`Name` as `item_category_parent_name`,
        good.`Specification` as `item_specification`,
        good.`Unit` as `item_unit`,
        billproduct.`ItemCnt` as `item_count`,
        billproduct.`PayAmount` as `item_pay_amount`,
        ifnull(case when billproduct.`PayAmount`=0 then 0 else billproduct.`PayAmount` / paybook.`bill_pay_amount` * paybook.`total_payment_amount` end, 0) as `item_payment_amount`,
        ifnull(case when billproduct.`PayAmount`=0 then 0 else billproduct.`PayAmount` / paybook.`total_payment_amount` * paybook.`total_real_payment_amount` end, 0) as `item_real_payment_amount`,
        ifnull(case when billproduct.`PayAmount`=0 then 0 else billproduct.`PayAmount` / paybook.`total_payment_amount` * paybook.`total_balance_payment_amount` end, 0) as `item_balance_payment_amount`,

        goodscategory.`ID` as `bill_classify_id`,
        goodscategory.`Name` as `bill_classify_name`,
        goodscategory.`ID` as `course_classify_id`,
        goodscategory.`Name` as `course_classify_name`,

         
            case when (DATE_FORMAT(paybook.`bill_pay_time`, '%Y-%m-%d') <= DATE_FORMAT(customer.`max_new_customer_bill_time`, '%Y-%m-%d')) then 1 else 0 end as `is_new_customer_bill`,
         
        case when paybook.`bill_id`=customer.`first_bill_id` then 1 else 0 end as `is_first_bill`,
        0 as `is_cool_sculpting`,
        bill.`source` as `bill_source`,
        bill.`in_app_purchase`
     
        from
            ( 
        select
            paybookdetail.`bill_detail_id`,
            paybookdetail.`bill_id`,
            paybookdetail.`bill_clinic_id`,
            paybookdetail.`bill_clinic_name`,
            paybookdetail.`customer_id`,
            paybookdetail.`visit_consumption_type`,
            paybookdetail.`visit_consultant_id`,
            paybookdetail.`visit_consultant_name`,
            paybookdetail.`bill_discounted_amount`,
            paybookdetail.`performers`,
            paybookdetail.`performers_details`,
            paybookdetail.`creator_id`,
            paybookdetail.`creator_name`,
            paybookdetail.`bill_create_time`,

            paybookdetail.`pay_book_id`,
            paybookdetail.`pay_clinic_id`,
            paybookdetail.`pay_clinic_name`,
            paybookdetail.`bill_pay_amount`,
            paybookdetail.`bill_pay_time`,
            group_concat(CONCAT(paybookchannel.`ChannelName`, ':', TRUNCATE((paybookchannel.`PayAmount` / 100), 2)) separator ';') as `payment_methods`,
            paybookdetail.`payment_check_off_code`,
            paybookdetail.`total_real_payment_amount`,
            paybookdetail.`total_balance_payment_amount`,
            paybookdetail.`total_payment_amount`
        from
            (
                select
                    paybookdetail.`BillDetailId` as `bill_detail_id`,
                    paybook.`bill_id`,
                    paybook.`bill_clinic_id`,
                    paybook.`bill_clinic_name`,
                    paybook.`customer_id`,
                    paybook.`visit_consumption_type`,
                    paybook.`visit_consultant_id`,
                    paybook.`visit_consultant_name`,
                    paybook.`bill_discounted_amount`,
                    paybook.`performers`,
                    paybook.`performers_details`,
                    paybook.`creator_id`,
                    paybook.`creator_name`,
                    paybook.`bill_create_time`,

                    paybook.`pay_book_id`,
                    paybook.`pay_clinic_id`,
                    paybook.`pay_clinic_name`,
                    paybook.`bill_pay_amount`,
                    paybook.`bill_pay_time`,
                    paybook.`payment_methods`,
                    paybook.`payment_check_off_code`,
                    sum(paybook.`total_real_payment_amount`) as `total_real_payment_amount`,
                    sum(paybook.`total_balance_payment_amount`) as `total_balance_payment_amount`,
                    sum(paybook.`total_payment_amount`) as `total_payment_amount`
                from
                    ( 
         
        select
            paybook.`bill_id`,
            paybook.`bill_clinic_id`,
            paybook.`bill_clinic_name`,
            paybook.`customer_id`,
            paybook.`visit_consumption_type`,
            paybook.`visit_consultant_id`,
            paybook.`visit_consultant_name`,
            paybook.`bill_discounted_amount`,
            paybook.`performers`,
            paybook.`performers_details`,
            paybook.`creator_id`,
            paybook.`creator_name`,
            paybook.`bill_create_time`,

            paybook.`pay_book_id`,
            paybook.`pay_clinic_id`,
            paybook.`pay_clinic_name`,
            paybook.`bill_pay_amount`,
            paybook.`bill_pay_time`,
            group_concat(CONCAT(paybookchannel.`ChannelName`, ':', TRUNCATE((paybookchannel.`PayAmount` / 100), 2)) separator ';') as `payment_methods`,
            group_concat(pay_book_channel_check_off_code.`check_off_code` separator ';') as `payment_check_off_code`,
            sum(case when paymentmethod.`IsRealConsumption`=1 or paybookchannel.`ChannelId`='6756de43-f712-c80b-9687-08d59e04cec0' then paybookchannel.`PayAmount` else 0 end) as `total_real_payment_amount`,
            sum(case when paybookchannel.`ChannelId`='6756de43-f712-c80b-9687-08d59e04cec0' then paybookchannel.`PayAmount` else 0 end) as `total_balance_payment_amount`,
            sum(case when paybookchannel.`ChannelId`='3379222d-235b-4065-b78e-fc39151b107c' then 0 else paybookchannel.`PayAmount` end) as `total_payment_amount`
        from
            (
                select
                    bill.`bill_id`,
                    bill.`bill_clinic_id`,
                    billclinic.`Name` as `bill_clinic_name`,
                    bill.`customer_id`,
                    bill.`visit_consumption_type`,
                    bill.`visit_consultant_id`,
                    bill.`visit_consultant_name`,
                    bill.`bill_discounted_amount`,
                    bill.`performers`,
                    bill.`performers_details`,
                    bill.`creator_id`,
                    bill.`creator_name`,
                    bill.`bill_create_time`,

                    paybook.`ID` as `pay_book_id`,
                    paybook.`OrganizationId` as `pay_clinic_id`,
                    payclinic.`Name` as `pay_clinic_name`,
                    paybook.`PayAmount` as `bill_pay_amount`,
                    paybook.`PayTime` as `bill_pay_time`
                from
                    (
                        select
                            bill.`bill_id`,
                            bill.`bill_clinic_id`,
                            bill.`customer_id`,
                            case visit.`Type` when 1 then '初诊' when 2 then '复诊' when 3 then '疗程内' else '再消费' end as `visit_consumption_type`,
                            visit.`ConsultantId` as `visit_consultant_id`,
                            visit.`ConsultantName` as `visit_consultant_name`,
                            bill.`bill_discounted_amount`,
                            group_concat(employee.`EmpName` separator ',') as `performers`,
                            group_concat(CONCAT(employee.`ID`, '|', employee.`EmpName`) separator ';') as `performers_details`,
                            bill.`creator_id`,
                            creator.`EmpName` as `creator_name`,
                            bill.`bill_create_time`
                        from
                            (
     
         
        select
            bill.`ID` as `bill_id`,
            bill.`ClinicId` as `bill_clinic_id`,
            bill.`Customer_ID` as `customer_id`,
            bill.`Visit_ID` as `bill_visit_id`,
            bill.`DiscountedAmount` as `bill_discounted_amount`,
            bill.`CreateEmpId` as `creator_id`,
            bill.`CreateTime` as `bill_create_time`
        from
             
        paybooks paybook
        left join bills bill on bill.`ID`=paybook.`BillId`
        left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
        left join customers customer on customer.`ID`=bill.`Customer_ID`
        left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
         
         
         
         
     
             
         WHERE  paybook.`CompId`=@companieId
            
            
                and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                
            
            and bill.`BillStatus`!=4 
     
        group by
            bill.`ID`
     
         
                            ) bill
                            left join billemployees billemployee on billemployee.`BillId`=bill.`bill_id`
                            left join employees employee on employee.`ID`=billemployee.`EmpId`
                            left join visits visit on visit.`ID`=bill.`bill_visit_id`
                            left join employees creator on creator.`ID`=bill.`creator_id`
                        group by
                            bill.`bill_id`
                    ) bill
                    left join paybooks paybook on paybook.`BillId`=bill.`bill_id`
                    left join organizations billclinic on billclinic.`ID`=bill.`bill_clinic_id`
                    left join organizations payclinic on payclinic.`ID`=paybook.`OrganizationId`
     
         WHERE  DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay 
         
            ) paybook
            left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`pay_book_id`
            left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
            left join pay_book_channel_check_off_codes pay_book_channel_check_off_code on pay_book_channel_check_off_code.`pay_book_channel_id`=paybookchannel.`ID`
        group by
            paybook.`pay_book_id`
     
     ) paybook
                    left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`pay_book_id`
                group by
                    paybookdetail.`BillDetailId`
            ) paybookdetail
            left join paybooks paybook on paybook.`BillId`=paybookdetail.`bill_id`
            left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`ID`
            left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
        group by
            paybookdetail.`bill_detail_id`
     ) paybook
            left join ( 
        select
            billdetail.`ID` as `bill_detail_id`,
            billdetail.`ItemId` as `item_id`,
            billdetail.`ItemName` as `item_name`,
            billdetail.`CareType` as `care_type`,
            billdetail.`Bill_ID` as `bill_id`,
            billdetail.`is_network_order`,
            case billdetail.`CareType` when 2 then (select promotion.`is_promotion` from promotions promotion where promotion.`ID`=billdetail.`ItemId`) else 0 end as `is_promotion`
        from
             
        paybooks paybook
        left join bills bill on bill.`ID`=paybook.`BillId`
        left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
        left join customers customer on customer.`ID`=bill.`Customer_ID`
        left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
         
         
         
         
     
             
         WHERE  paybook.`CompId`=@companieId
            
            
                and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                
            
            and bill.`BillStatus`!=4 
     
        group by
            billdetail.`ID`
     ) billdetail on billdetail.`bill_detail_id`=paybook.`bill_detail_id`
            left join bills bill on bill.`ID`=billdetail.`bill_id`
            left join ( 
         
        select
            customer.*,
            customercard.`CardID` as `customer_card_id`,
            vipcard.`CardName` as `customer_card_name`,

            coursedetail.`ID` as `last_course_detail_id`,
            cc.`ServiceId` as `last_course_detail_service_id`,
            careservice.`Name` as `last_course_detail_service_name`,
            coursedetail.`CourseCount` as `last_course_detail_count`,
            coursedetail.`CourseAmount` as `last_course_detail_amount`
        from
            (
                select
                    customer.*,

                    max(coursedetail.`CreateTime`) as `last_course_detail_time`
                from
                    (
                        select
                            customer.*,

                            bill.`ID` as `first_bill_id`,
                            bill.`DiscountedAmount` as `first_discounted_amount`,
                            adddate(customer.`first_bill_time`, INTERVAL 30 DAY) as `max_new_customer_bill_time`
                        from
                            (
                                select
                                    customer.`customer_id`,
                                    customer.`customer_name`,
                                    customer.`customer_sex`,
                                    customer.`customer_cid`,
                                    customer.`customer_create_time`,
                                    customer.`max_new_customer_time`,
                                    customer.`customer_source_id`,
                                    customer.`customer_source_name`,
                                    customer.`customer_source_parent_id`,
                                    customer.`customer_stakeholder_id`,
                                    customer.`customer_stakeholder_name`,
                                    customer.`nurse_id`,
                                    customer.`nurse_name`,
                                    min(bill.`CreateTime`) as `first_bill_time`
                                from
                                    (
                                        select
                                            currentcustomer.`customer_id`,
                                            customer.`Name` as `customer_name`,
                                            case customer.`Sex` when 1 then '男' when 2 then '女' else '' end as `customer_sex`,
                                            customer.`CID` as `customer_cid`,
                                            customer.`CreatTime` as `customer_create_time`,
                                            adddate(customer.`CreatTime`, INTERVAL 1 YEAR) as `max_new_customer_time`,
                                            source.`ID` as `customer_source_id`,
                                            source.`Name` as `customer_source_name`,
                                            source.`ParentID` as `customer_source_parent_id`,
                                            customer.`EquityId` as `customer_stakeholder_id`,
                                            stakeholder.`EmpName` as `customer_stakeholder_name`,
                                            customer.`CustomerServerID` as `nurse_id`,
                                            nurse.`EmpName` as `nurse_name`
                                        from
                                            (
                                                select
                                                    bill.`Customer_ID` as `customer_id`
                                                from
                                                    (
     
         
        select
            bill.`ID` as `bill_id`,
            bill.`ClinicId` as `bill_clinic_id`,
            bill.`Customer_ID` as `customer_id`,
            bill.`Visit_ID` as `bill_visit_id`,
            bill.`DiscountedAmount` as `bill_discounted_amount`,
            bill.`CreateEmpId` as `creator_id`,
            bill.`CreateTime` as `bill_create_time`
        from
             
        paybooks paybook
        left join bills bill on bill.`ID`=paybook.`BillId`
        left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
        left join customers customer on customer.`ID`=bill.`Customer_ID`
        left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
         
         
         
         
     
             
         WHERE  paybook.`CompId`=@companieId
            
            
                and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                
            
            and bill.`BillStatus`!=4 
     
        group by
            bill.`ID`
     
         
                                                    ) bill
                                                group by
                                                    bill.`Customer_ID`
                                            ) currentcustomer
                                            left join customers customer on customer.`ID`=currentcustomer.`customer_id`
                                            left join sources source on source.`ID`=customer.`LaiYuanID`
                                            left join employees stakeholder on stakeholder.`ID`=customer.`EquityId`
                                            left join employees nurse on nurse.`ID`=customer.`CustomerServerID`
                                    ) customer
                                    left join bills bill on bill.`Customer_ID`=customer.`customer_id`
                                 WHERE  bill.`BillStatus`!=4
                                    
                                        and DATE_FORMAT(bill.`CreateTime`, '%Y-%m-%d')<=@endDay 
                                group by
                                    customer.`customer_id`
                            ) customer
                            left join bills bill on bill.`Customer_ID`=customer.`customer_id`
                        where
                            bill.`CreateTime`=customer.`first_bill_time`
                    ) customer
                    left join courses course on course.`Customer_ID`=customer.`customer_id`
                    left join (
                        select
                            coursedetail.*
                        from
                            coursedetails coursedetail
                        where
                             
                                DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')<=@endDay
                             
                    ) coursedetail on coursedetail.`Course_ID`=course.`ID`
                group by
                    customer.`customer_id`
            ) customer
            left join customercards customercard on customercard.`ID`=customer.`customer_id`
            left join vipcards vipcard on vipcard.`ID`=customercard.`CardID`
            left join courses course on course.`Customer_ID`=customer.`customer_id`
            left join coursedetails coursedetail on coursedetail.`Course_ID`=course.`ID`
            left join courses cc on cc.`ID`=coursedetail.`Course_ID`
            left join careservices careservice on careservice.`ID`=cc.`ServiceId`
        where
            if(customer.`last_course_detail_time` is null, 1=1, coursedetail.`CreateTime`=customer.`last_course_detail_time`)
        group by
            customer.`customer_id`
     
     ) customer on customer.`customer_id`=bill.`Customer_ID`
            left join billproducts billproduct on billproduct.`BillDetailiId`=billdetail.`bill_detail_id`
            left join goods good on good.`ID`=billproduct.`GoodsId`
            left join goodscategorys goodscategory on goodscategory.`ID`=good.`GoodsCategory_ID`
            left join goodscategorys goodscategoryparent on goodscategoryparent.`ID`=goodscategory.`ParentCategory_ID`
         WHERE  billproduct.`BillDetailiId` is not null 
     
            union all
             
        select
             
        paybook.`pay_book_id`,
        paybook.`bill_id`,
        paybook.`bill_clinic_id`,
        paybook.`bill_clinic_name`,
        paybook.`pay_clinic_id`,
        paybook.`pay_clinic_name`,
        paybook.`visit_consumption_type`,
        paybook.`visit_consultant_id`,
        paybook.`visit_consultant_name`,
        paybook.`payment_methods`,
        ifnull(paybook.`payment_check_off_code`,'') as `payment_check_off_code`,
        paybook.`bill_discounted_amount` as `bill_discounted_amount`,
        paybook.`bill_pay_amount` as `bill_pay_amount`,
        paybook.`total_payment_amount` as `bill_payment_amount`,
        paybook.`total_real_payment_amount` as `bill_real_payment_amount`,
        paybook.`total_balance_payment_amount` as `bill_balance_payment_amount`,
        paybook.`creator_id`,
        paybook.`creator_name`,
        paybook.`performers`,
        paybook.`performers_details`,
        paybook.`bill_create_time`,
        paybook.`bill_pay_time`,
     
             
        customer.`customer_id`,
        customer.`customer_name`,
        customer.`customer_cid`,
        customer.`customer_sex`,
        customer.`customer_card_id`,
        customer.`customer_card_name`,
        customer.`customer_source_id`,
        customer.`customer_source_name`,
        customer.`customer_source_parent_id`,
        customer.`first_bill_time`,
        customer.`last_course_detail_time`,
        customer.`customer_stakeholder_id`,
        customer.`customer_stakeholder_name`,
        customer.`nurse_id`,
        customer.`nurse_name`,
        customer.`max_new_customer_bill_time`,
        customer.`max_new_customer_time`,
     
             
        billdetail.`bill_detail_id`,
        billdetail.`item_id` as `item_source_id`,
        billdetail.`item_name` as `item_source_name`,
        billdetail.`care_type` as `item_source_type`,
        case billdetail.`care_type` when 0 then '服务项目' when 1 then '商品' else '促销活动' end as `item_source_type_name`,
        case billdetail.`care_type` when 0 then (select ifnull(cs.`IsGroupItem`, 0) as `item_source_service_is_group` from careservices cs where cs.`ID`=billdetail.`item_id`) else 0 end as `item_source_service_is_group`,
        billdetail.`is_network_order`,
        billdetail.`is_promotion`,
        ifnull(bill.`Memo`,'') as `bill_memo`,
     
             
        case when paybook.`bill_id`=customer.`first_bill_id` then ifnull((
            select
                sum(pbc.`PayAmount`) as `first_day_total_real_payment_amount`
            from
                paybooks pb
                left join bills b on b.`ID`=pb.`BillId`
                left join paybookchannels pbc on pbc.`PayBookId`=pb.`ID`
                left join paymentmethods pt on pt.`ID`=pbc.`ChannelId`
            where
                b.`Customer_ID`=customer.`customer_id`
                and DATE_FORMAT(pb.`PayTime`, '%Y-%m-%d')=DATE_FORMAT(customer.`first_bill_time`, '%Y-%m-%d')
                and (pt.`IsRealConsumption`=1 or pt.`ID`='6756de43-f712-c80b-9687-08d59e04cec0')
            group by
                b.`Customer_ID`
        ), 0) else 0 end as `first_day_total_real_payment_amount`,
     
             
        0 as `item_type`,
        '项目' as `item_type_name`,

        course.`ID` as `bill_item_id`,
        careservice.`ID` as `item_id`,
        careservice.`Name` as `item_name`,
        itemcategory.`ID` as `item_category_id`,
        itemcategory.`Name` as `item_category_name`,
        itemparentcategory.`ID` as `item_category_parent_id`,
        itemparentcategory.`Name` as `item_category_parent_name`,
        '' as `item_specification`,
        '' as `item_unit`,
        course.`SumCourseQty` / (ifnull(careservice.`CourseQty`,1) + ifnull(careservice.`GiftQty`,0)) as `item_count`,
        course.`SumAmount` as `item_pay_amount`,
        ifnull(case when course.`SumAmount`=0 then 0 else course.`SumAmount` / paybook.`bill_pay_amount` * paybook.`total_payment_amount` end, 0) as `item_payment_amount`,
        ifnull(case when course.`SumAmount`=0 then 0 else course.`SumAmount` / paybook.`total_payment_amount` * paybook.`total_real_payment_amount` end, 0) as `item_real_payment_amount`,
        ifnull(case when course.`SumAmount`=0 then 0 else course.`SumAmount` / paybook.`total_payment_amount` * paybook.`total_balance_payment_amount` end, 0) as `item_balance_payment_amount`,

        careservice.`bill_classify_id`,
        (select csbc.`Name` from care_services_bill_classifys csbc where csbc.`ID`=careservice.`bill_classify_id`) as `bill_classify_name`,
        careservice.`course_classify_id`,
        (select cscc.`Name` from care_services_course_classifys cscc where cscc.`ID`=careservice.`course_classify_id`) as `course_classify_name`,

         
            case when (DATE_FORMAT(paybook.`bill_pay_time`, '%Y-%m-%d') <= DATE_FORMAT(customer.`max_new_customer_bill_time`, '%Y-%m-%d')) then 1 else 0 end as `is_new_customer_bill`,
         
        case when paybook.`bill_id`=customer.`first_bill_id` then 1 else 0 end as `is_first_bill`,
        case when careservice.`bill_classify_id`='86ca40ac-d533-405e-017a-8f5313cd207f' or careservice.`bill_classify_id`='6ac32a4b-6e44-900f-5477-69a943a30706' then 1 else 0 end as `is_cool_sculpting`,
        bill.`source` as `bill_source`,
        bill.`in_app_purchase`
     
        from
            ( 
        select
            paybookdetail.`bill_detail_id`,
            paybookdetail.`bill_id`,
            paybookdetail.`bill_clinic_id`,
            paybookdetail.`bill_clinic_name`,
            paybookdetail.`customer_id`,
            paybookdetail.`visit_consumption_type`,
            paybookdetail.`visit_consultant_id`,
            paybookdetail.`visit_consultant_name`,
            paybookdetail.`bill_discounted_amount`,
            paybookdetail.`performers`,
            paybookdetail.`performers_details`,
            paybookdetail.`creator_id`,
            paybookdetail.`creator_name`,
            paybookdetail.`bill_create_time`,

            paybookdetail.`pay_book_id`,
            paybookdetail.`pay_clinic_id`,
            paybookdetail.`pay_clinic_name`,
            paybookdetail.`bill_pay_amount`,
            paybookdetail.`bill_pay_time`,
            group_concat(CONCAT(paybookchannel.`ChannelName`, ':', TRUNCATE((paybookchannel.`PayAmount` / 100), 2)) separator ';') as `payment_methods`,
            paybookdetail.`payment_check_off_code`,
            paybookdetail.`total_real_payment_amount`,
            paybookdetail.`total_balance_payment_amount`,
            paybookdetail.`total_payment_amount`
        from
            (
                select
                    paybookdetail.`BillDetailId` as `bill_detail_id`,
                    paybook.`bill_id`,
                    paybook.`bill_clinic_id`,
                    paybook.`bill_clinic_name`,
                    paybook.`customer_id`,
                    paybook.`visit_consumption_type`,
                    paybook.`visit_consultant_id`,
                    paybook.`visit_consultant_name`,
                    paybook.`bill_discounted_amount`,
                    paybook.`performers`,
                    paybook.`performers_details`,
                    paybook.`creator_id`,
                    paybook.`creator_name`,
                    paybook.`bill_create_time`,

                    paybook.`pay_book_id`,
                    paybook.`pay_clinic_id`,
                    paybook.`pay_clinic_name`,
                    paybook.`bill_pay_amount`,
                    paybook.`bill_pay_time`,
                    paybook.`payment_methods`,
                    paybook.`payment_check_off_code`,
                    sum(paybook.`total_real_payment_amount`) as `total_real_payment_amount`,
                    sum(paybook.`total_balance_payment_amount`) as `total_balance_payment_amount`,
                    sum(paybook.`total_payment_amount`) as `total_payment_amount`
                from
                    ( 
         
        select
            paybook.`bill_id`,
            paybook.`bill_clinic_id`,
            paybook.`bill_clinic_name`,
            paybook.`customer_id`,
            paybook.`visit_consumption_type`,
            paybook.`visit_consultant_id`,
            paybook.`visit_consultant_name`,
            paybook.`bill_discounted_amount`,
            paybook.`performers`,
            paybook.`performers_details`,
            paybook.`creator_id`,
            paybook.`creator_name`,
            paybook.`bill_create_time`,

            paybook.`pay_book_id`,
            paybook.`pay_clinic_id`,
            paybook.`pay_clinic_name`,
            paybook.`bill_pay_amount`,
            paybook.`bill_pay_time`,
            group_concat(CONCAT(paybookchannel.`ChannelName`, ':', TRUNCATE((paybookchannel.`PayAmount` / 100), 2)) separator ';') as `payment_methods`,
            group_concat(pay_book_channel_check_off_code.`check_off_code` separator ';') as `payment_check_off_code`,
            sum(case when paymentmethod.`IsRealConsumption`=1 or paybookchannel.`ChannelId`='6756de43-f712-c80b-9687-08d59e04cec0' then paybookchannel.`PayAmount` else 0 end) as `total_real_payment_amount`,
            sum(case when paybookchannel.`ChannelId`='6756de43-f712-c80b-9687-08d59e04cec0' then paybookchannel.`PayAmount` else 0 end) as `total_balance_payment_amount`,
            sum(case when paybookchannel.`ChannelId`='3379222d-235b-4065-b78e-fc39151b107c' then 0 else paybookchannel.`PayAmount` end) as `total_payment_amount`
        from
            (
                select
                    bill.`bill_id`,
                    bill.`bill_clinic_id`,
                    billclinic.`Name` as `bill_clinic_name`,
                    bill.`customer_id`,
                    bill.`visit_consumption_type`,
                    bill.`visit_consultant_id`,
                    bill.`visit_consultant_name`,
                    bill.`bill_discounted_amount`,
                    bill.`performers`,
                    bill.`performers_details`,
                    bill.`creator_id`,
                    bill.`creator_name`,
                    bill.`bill_create_time`,

                    paybook.`ID` as `pay_book_id`,
                    paybook.`OrganizationId` as `pay_clinic_id`,
                    payclinic.`Name` as `pay_clinic_name`,
                    paybook.`PayAmount` as `bill_pay_amount`,
                    paybook.`PayTime` as `bill_pay_time`
                from
                    (
                        select
                            bill.`bill_id`,
                            bill.`bill_clinic_id`,
                            bill.`customer_id`,
                            case visit.`Type` when 1 then '初诊' when 2 then '复诊' when 3 then '疗程内' else '再消费' end as `visit_consumption_type`,
                            visit.`ConsultantId` as `visit_consultant_id`,
                            visit.`ConsultantName` as `visit_consultant_name`,
                            bill.`bill_discounted_amount`,
                            group_concat(employee.`EmpName` separator ',') as `performers`,
                            group_concat(CONCAT(employee.`ID`, '|', employee.`EmpName`) separator ';') as `performers_details`,
                            bill.`creator_id`,
                            creator.`EmpName` as `creator_name`,
                            bill.`bill_create_time`
                        from
                            (
     
         
        select
            bill.`ID` as `bill_id`,
            bill.`ClinicId` as `bill_clinic_id`,
            bill.`Customer_ID` as `customer_id`,
            bill.`Visit_ID` as `bill_visit_id`,
            bill.`DiscountedAmount` as `bill_discounted_amount`,
            bill.`CreateEmpId` as `creator_id`,
            bill.`CreateTime` as `bill_create_time`
        from
             
        paybooks paybook
        left join bills bill on bill.`ID`=paybook.`BillId`
        left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
        left join customers customer on customer.`ID`=bill.`Customer_ID`
        left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
         
         
         
         
     
             
         WHERE  paybook.`CompId`=@companieId
            
            
                and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                
            
            and bill.`BillStatus`!=4 
     
        group by
            bill.`ID`
     
         
                            ) bill
                            left join billemployees billemployee on billemployee.`BillId`=bill.`bill_id`
                            left join employees employee on employee.`ID`=billemployee.`EmpId`
                            left join visits visit on visit.`ID`=bill.`bill_visit_id`
                            left join employees creator on creator.`ID`=bill.`creator_id`
                        group by
                            bill.`bill_id`
                    ) bill
                    left join paybooks paybook on paybook.`BillId`=bill.`bill_id`
                    left join organizations billclinic on billclinic.`ID`=bill.`bill_clinic_id`
                    left join organizations payclinic on payclinic.`ID`=paybook.`OrganizationId`
     
         WHERE  DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay 
         
            ) paybook
            left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`pay_book_id`
            left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
            left join pay_book_channel_check_off_codes pay_book_channel_check_off_code on pay_book_channel_check_off_code.`pay_book_channel_id`=paybookchannel.`ID`
        group by
            paybook.`pay_book_id`
     
     ) paybook
                    left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`pay_book_id`
                group by
                    paybookdetail.`BillDetailId`
            ) paybookdetail
            left join paybooks paybook on paybook.`BillId`=paybookdetail.`bill_id`
            left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`ID`
            left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
        group by
            paybookdetail.`bill_detail_id`
     ) paybook
            left join ( 
        select
            billdetail.`ID` as `bill_detail_id`,
            billdetail.`ItemId` as `item_id`,
            billdetail.`ItemName` as `item_name`,
            billdetail.`CareType` as `care_type`,
            billdetail.`Bill_ID` as `bill_id`,
            billdetail.`is_network_order`,
            case billdetail.`CareType` when 2 then (select promotion.`is_promotion` from promotions promotion where promotion.`ID`=billdetail.`ItemId`) else 0 end as `is_promotion`
        from
             
        paybooks paybook
        left join bills bill on bill.`ID`=paybook.`BillId`
        left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
        left join customers customer on customer.`ID`=bill.`Customer_ID`
        left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
         
         
         
         
     
             
         WHERE  paybook.`CompId`=@companieId
            
            
                and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                
            
            and bill.`BillStatus`!=4 
     
        group by
            billdetail.`ID`
     ) billdetail on billdetail.`bill_detail_id`=paybook.`bill_detail_id`
            left join bills bill on bill.`ID`=billdetail.`bill_id`
            left join ( 
         
        select
            customer.*,
            customercard.`CardID` as `customer_card_id`,
            vipcard.`CardName` as `customer_card_name`,

            coursedetail.`ID` as `last_course_detail_id`,
            cc.`ServiceId` as `last_course_detail_service_id`,
            careservice.`Name` as `last_course_detail_service_name`,
            coursedetail.`CourseCount` as `last_course_detail_count`,
            coursedetail.`CourseAmount` as `last_course_detail_amount`
        from
            (
                select
                    customer.*,

                    max(coursedetail.`CreateTime`) as `last_course_detail_time`
                from
                    (
                        select
                            customer.*,

                            bill.`ID` as `first_bill_id`,
                            bill.`DiscountedAmount` as `first_discounted_amount`,
                            adddate(customer.`first_bill_time`, INTERVAL 30 DAY) as `max_new_customer_bill_time`
                        from
                            (
                                select
                                    customer.`customer_id`,
                                    customer.`customer_name`,
                                    customer.`customer_sex`,
                                    customer.`customer_cid`,
                                    customer.`customer_create_time`,
                                    customer.`max_new_customer_time`,
                                    customer.`customer_source_id`,
                                    customer.`customer_source_name`,
                                    customer.`customer_source_parent_id`,
                                    customer.`customer_stakeholder_id`,
                                    customer.`customer_stakeholder_name`,
                                    customer.`nurse_id`,
                                    customer.`nurse_name`,
                                    min(bill.`CreateTime`) as `first_bill_time`
                                from
                                    (
                                        select
                                            currentcustomer.`customer_id`,
                                            customer.`Name` as `customer_name`,
                                            case customer.`Sex` when 1 then '男' when 2 then '女' else '' end as `customer_sex`,
                                            customer.`CID` as `customer_cid`,
                                            customer.`CreatTime` as `customer_create_time`,
                                            adddate(customer.`CreatTime`, INTERVAL 1 YEAR) as `max_new_customer_time`,
                                            source.`ID` as `customer_source_id`,
                                            source.`Name` as `customer_source_name`,
                                            source.`ParentID` as `customer_source_parent_id`,
                                            customer.`EquityId` as `customer_stakeholder_id`,
                                            stakeholder.`EmpName` as `customer_stakeholder_name`,
                                            customer.`CustomerServerID` as `nurse_id`,
                                            nurse.`EmpName` as `nurse_name`
                                        from
                                            (
                                                select
                                                    bill.`Customer_ID` as `customer_id`
                                                from
                                                    (
     
         
        select
            bill.`ID` as `bill_id`,
            bill.`ClinicId` as `bill_clinic_id`,
            bill.`Customer_ID` as `customer_id`,
            bill.`Visit_ID` as `bill_visit_id`,
            bill.`DiscountedAmount` as `bill_discounted_amount`,
            bill.`CreateEmpId` as `creator_id`,
            bill.`CreateTime` as `bill_create_time`
        from
             
        paybooks paybook
        left join bills bill on bill.`ID`=paybook.`BillId`
        left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
        left join customers customer on customer.`ID`=bill.`Customer_ID`
        left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
         
         
         
         
     
             
         WHERE  paybook.`CompId`=@companieId
            
            
                and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                
            
            
                
                    and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                
            
            and bill.`BillStatus`!=4 
     
        group by
            bill.`ID`
     
         
                                                    ) bill
                                                group by
                                                    bill.`Customer_ID`
                                            ) currentcustomer
                                            left join customers customer on customer.`ID`=currentcustomer.`customer_id`
                                            left join sources source on source.`ID`=customer.`LaiYuanID`
                                            left join employees stakeholder on stakeholder.`ID`=customer.`EquityId`
                                            left join employees nurse on nurse.`ID`=customer.`CustomerServerID`
                                    ) customer
                                    left join bills bill on bill.`Customer_ID`=customer.`customer_id`
                                 WHERE  bill.`BillStatus`!=4
                                    
                                        and DATE_FORMAT(bill.`CreateTime`, '%Y-%m-%d')<=@endDay 
                                group by
                                    customer.`customer_id`
                            ) customer
                            left join bills bill on bill.`Customer_ID`=customer.`customer_id`
                        where
                            bill.`CreateTime`=customer.`first_bill_time`
                    ) customer
                    left join courses course on course.`Customer_ID`=customer.`customer_id`
                    left join (
                        select
                            coursedetail.*
                        from
                            coursedetails coursedetail
                        where
                             
                                DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')<=@endDay
                             
                    ) coursedetail on coursedetail.`Course_ID`=course.`ID`
                group by
                    customer.`customer_id`
            ) customer
            left join customercards customercard on customercard.`ID`=customer.`customer_id`
            left join vipcards vipcard on vipcard.`ID`=customercard.`CardID`
            left join courses course on course.`Customer_ID`=customer.`customer_id`
            left join coursedetails coursedetail on coursedetail.`Course_ID`=course.`ID`
            left join courses cc on cc.`ID`=coursedetail.`Course_ID`
            left join careservices careservice on careservice.`ID`=cc.`ServiceId`
        where
            if(customer.`last_course_detail_time` is null, 1=1, coursedetail.`CreateTime`=customer.`last_course_detail_time`)
        group by
            customer.`customer_id`
     
     ) customer on customer.`customer_id`=bill.`Customer_ID`
            left join courses course on course.`BillDetailiId`=billdetail.`bill_detail_id`
            left join careservices careservice on careservice.`ID`=course.`ServiceId`
            left join itemcategorys itemcategory on itemcategory.`ID`=careservice.`Category_ID`
            left join itemcategorys itemparentcategory on itemparentcategory.`ID`=itemcategory.`ParentId`
         WHERE  course.`BillDetailiId` is not null 
     
         
     ) billsdetail
     left join (
        select
          t.`customer_id`,
          min(paybook.`PayTime`) as `first_real_pay_time`
        from
          (
            select
            bill.`Customer_ID`
            from paybooks paybook
            left join bills bill on bill.`ID`=paybook.`BillId`
            left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
            left join customers customer on customer.`ID`=bill.`Customer_ID`
            left join paybookdetails paybookdetail on paybookdetail.`PayBookId`=paybook.`ID`
            WHERE  paybook.`CompId`=@companieId
            and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
            and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
            and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
            and bill.`BillStatus`!=4 
            group by bill.`Customer_ID`
          ) t
        left join bills bill on bill.`Customer_ID`=t.`customer_id`
        left join paybooks paybook on paybook.`BillId`=bill.`ID`
        left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`ID`
        left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
      where
        paymentmethod.`IsRealConsumption`=1 or paymentmethod.`ID`='6756de43-f712-c80b-9687-08d59e04cec0'
      group by
        t.`customer_id`
     ) customer_exitr on customer_exitr.`customer_id`=billsdetail.`customer_id`
     left join (
select
billproduct.`ID` as `bill_item_id`,
sum(billproduct.`refund_real_amount`) as `refund_real_amount`
from paybooks paybook
left join bills bill on bill.`ID`=paybook.`BillId`
left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
left join billproducts billproduct on billproduct.`BillDetailiId`=billdetail.`ID`
WHERE  paybook.`CompId`=@companieId
and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
and bill.`BillStatus`!=4
and billproduct.`refund_real_amount`>0
group by billproduct.`ID`

union all

select
course.`ID` as `bill_item_id`,
sum(course.`refund_real_amount`) as `refund_real_amount`
from paybooks paybook
left join bills bill on bill.`ID`=paybook.`BillId`
left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
left join courses course on course.`BillDetailiId`=billdetail.`ID`
WHERE  paybook.`CompId`=@companieId
and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
and bill.`BillStatus`!=4
and course.`refund_real_amount`>0
group by course.`ID`
     ) refunddetail on refunddetail.`bill_item_id`=billsdetail.`bill_item_id`
          
        order by
        billsdetail.`bill_pay_time` desc